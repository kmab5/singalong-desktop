//! Native-selected paths never cross IPC. Writes replace only the selected destination.
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
#[cfg(windows)]
pub fn choose(
    save: bool,
    diagnostics: bool,
    cancel: Option<crate::platform::Cancellation>,
) -> Result<Option<PathBuf>, String> {
    use std::os::windows::ffi::OsStringExt;
    use windows::{
        Win32::{
            System::Com::{
                CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
                CoTaskMemFree, CoUninitialize,
            },
            UI::Shell::{
                Common::COMDLG_FILTERSPEC, FOS_DONTADDTORECENT, FOS_FILEMUSTEXIST,
                FOS_FORCEFILESYSTEM, FOS_OVERWRITEPROMPT, FOS_PATHMUSTEXIST, FileOpenDialog,
                FileSaveDialog, IFileDialog, SIGDN_FILESYSPATH,
            },
        },
        core::w,
    };
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe {
                CoUninitialize();
            }
        }
    }
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)
            .ok()
            .map_err(|_| "Could not open the file picker.")?;
        let _apartment = Apartment;
        let dialog: IFileDialog = CoCreateInstance(
            if save {
                &FileSaveDialog
            } else {
                &FileOpenDialog
            },
            None,
            CLSCTX_INPROC_SERVER,
        )
        .map_err(|_| "Could not open the file picker.")?;
        dialog
            .SetOptions(
                FOS_FORCEFILESYSTEM
                    | FOS_PATHMUSTEXIST
                    | FOS_DONTADDTORECENT
                    | if save {
                        FOS_OVERWRITEPROMPT
                    } else {
                        FOS_FILEMUSTEXIST
                    },
            )
            .map_err(|_| "Could not configure the file picker.")?;
        let filters = if diagnostics {
            vec![COMDLG_FILTERSPEC {
                pszName: w!("Diagnostics JSON"),
                pszSpec: w!("*.json"),
            }]
        } else if save {
            vec![COMDLG_FILTERSPEC {
                pszName: w!("Singalong desktop backup"),
                pszSpec: w!("*.json"),
            }]
        } else {
            vec![COMDLG_FILTERSPEC {
                pszName: w!("Singalong backups"),
                pszSpec: w!("*.json;*.zip"),
            }]
        };
        dialog
            .SetFileTypes(&filters)
            .map_err(|_| "Could not configure the file picker.")?;
        if save {
            dialog
                .SetFileName(if diagnostics {
                    w!("singalong-diagnostics.json")
                } else {
                    w!("singalong-backup.json")
                })
                .map_err(|_| "Could not configure the save dialog.")?;
            dialog
                .SetDefaultExtension(w!("json"))
                .map_err(|_| "Could not configure the save dialog.")?;
        }
        let _timer = dialog_cancel::Watch::new(&dialog, cancel)?;
        if let Err(error) = dialog.Show(None) {
            if error.code().0 as u32 == 0x800704c7 {
                return Ok(None);
            }
            return Err("The file picker could not finish.".into());
        }
        let item = dialog.GetResult().map_err(|_| "No file was selected.")?;
        let name = item
            .GetDisplayName(SIGDN_FILESYSPATH)
            .map_err(|_| "Choose a local file.")?;
        let path = PathBuf::from(std::ffi::OsString::from_wide(name.as_wide()));
        CoTaskMemFree(Some(name.0.cast()));
        Ok(Some(path))
    }
}
#[cfg(not(windows))]
pub fn choose(
    _: bool,
    _: bool,
    _: Option<crate::platform::Cancellation>,
) -> Result<Option<PathBuf>, String> {
    Err("Native file selection is available on Windows.".into())
}
pub fn atomic_write(
    path: &Path,
    write: impl FnOnce(&mut File) -> Result<(), String>,
    cancelled: impl Fn() -> bool,
) -> Result<(), String> {
    let parent = path.parent().ok_or("Choose a destination folder.")?;
    let temporary = parent.join(format!(".singalong-{}.tmp", uuid::Uuid::new_v4()));
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _cleanup = Cleanup(temporary.clone());
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|_| "Could not create the backup destination.")?;
    if cancelled() {
        return Err("Operation cancelled.".into());
    }
    write(&mut file)?;
    file.flush()
        .and_then(|_| file.sync_all())
        .map_err(|_| "Could not finish writing the selected file.")?;
    drop(file);
    if cancelled() {
        return Err("Operation cancelled.".into());
    }
    replace(&temporary, path)
}
#[cfg(windows)]
fn replace(from: &Path, to: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        Win32::Storage::FileSystem::{
            MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
        },
        core::PCWSTR,
    };
    let from: Vec<_> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<_> = to.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
        MoveFileExW(
            PCWSTR(from.as_ptr()),
            PCWSTR(to.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(|_| "Could not replace the selected destination; the previous file was kept.".into())
}
#[cfg(not(windows))]
fn replace(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::rename(from, to).map_err(|_| "Could not replace the selected destination.".into())
}

#[cfg(windows)]
mod dialog_cancel {
    use crate::platform::Cancellation;
    use std::cell::RefCell;
    use windows::{
        Win32::{
            Foundation::HWND,
            UI::{
                Shell::IFileDialog,
                WindowsAndMessaging::{KillTimer, SetTimer},
            },
        },
        core::HRESULT,
    };
    thread_local! {static CURRENT:RefCell<Option<(IFileDialog,Cancellation)>>=const{RefCell::new(None)};}
    pub struct Watch(usize);
    impl Watch {
        pub fn new(dialog: &IFileDialog, cancel: Option<Cancellation>) -> Result<Self, String> {
            let Some(cancel) = cancel else {
                return Ok(Self(0));
            };
            if cancel.cancelled() {
                return Err("Backup operation cancelled.".into());
            }
            let timer = unsafe { SetTimer(None, 0, 100, Some(tick)) };
            if timer == 0 {
                return Err("Could not initialize cancellable file selection.".into());
            }
            CURRENT.with(|slot| *slot.borrow_mut() = Some((dialog.clone(), cancel)));
            Ok(Self(timer))
        }
    }
    unsafe extern "system" fn tick(_: HWND, _: u32, _: usize, _: u32) {
        CURRENT.with(|slot| {
            if let Some((dialog, cancel)) = slot.borrow().as_ref()
                && cancel.cancelled()
            {
                let _ = unsafe { dialog.Close(HRESULT(0x800704c7u32 as i32)) };
            }
        });
    }
    impl Drop for Watch {
        fn drop(&mut self) {
            if self.0 != 0 {
                unsafe {
                    let _ = KillTimer(None, self.0);
                }
                CURRENT.with(|slot| {
                    slot.borrow_mut().take();
                });
            }
        }
    }
}
