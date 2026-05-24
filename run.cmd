@echo off
REM Same as start.cmd (kept for compatibility)
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0desk.ps1" -Action all
