@echo off
REM Compile AutoKey.exe (a lancer UNE fois, sur un PC Windows avec Python)
python -m pip install --upgrade pynput pyinstaller || goto :err
python -m PyInstaller --onefile --windowed --name AutoKey ^
  --hidden-import pynput.keyboard._win32 --hidden-import pynput.mouse._win32 ^
  autokey.py || goto :err
echo.
echo OK : dist\AutoKey.exe
pause
exit /b 0
:err
echo ECHEC de la compilation
pause
exit /b 1
