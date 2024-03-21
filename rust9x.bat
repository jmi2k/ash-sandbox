@echo off

set target=i586-rust9x-windows-msvc

cargo +rust9x build ^
    --target %target% ^
    --release ^
    || exit /b %ERRORLEVEL%

"C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.37.32822\bin\Hostx64\x86\editbin.exe" ^
    target\%target%\release\*.exe ^
    /SUBSYSTEM:WINDOWS,4.0 ^
    /RELEASE
