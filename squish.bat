@echo off

:: Bring the binary size to the minimum.

set target=x86_64-pc-windows-msvc

cargo +nightly build ^
    -Z build-std=std,panic_abort ^
    -Z build-std-features=panic_immediate_abort ^
    --target %target% ^
    --release ^
    || exit /b %ERRORLEVEL%

echo: >&2
echo This binary is almost impossible to debug, avoid distributing it! >&2
