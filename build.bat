@echo off
color 0B
echo ========================================================
echo   Compiling IBB-Hooker (x64 + x86)
echo ========================================================
echo.

echo [0/3] Checking and installing 32-bit (x86) support...
rustup target add i686-pc-windows-msvc

echo.
echo [1/3] Compiling Manager and Hook for x64...
cargo build --release --target x86_64-pc-windows-msvc
if %errorlevel% neq 0 exit /b %errorlevel%

echo.
echo [2/3] Compiling support Hook for x86 applications...
cargo build -p hook --release --target i686-pc-windows-msvc
if %errorlevel% neq 0 exit /b %errorlevel%

echo.
echo [3/3] Assembling final files into the "Dist" folder...
if not exist "Dist" mkdir "Dist"

copy /Y "target\x86_64-pc-windows-msvc\release\manager.exe" "Dist\manager.exe" >nul
copy /Y "target\x86_64-pc-windows-msvc\release\hook.dll" "Dist\hook-x64.dll" >nul
copy /Y "target\i686-pc-windows-msvc\release\hook.dll" "Dist\hook-x86.dll" >nul

echo.
color 0A
echo DONE! 
echo The complete and portable application is located in the "Dist" folder.
echo Open the Dist folder and run manager.exe!
echo.
pause