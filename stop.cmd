@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0desk.ps1" -Action stop
pause
