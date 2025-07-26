@echo off
REM Ensure this script is run from the root of a Git repository

echo Cleaning up local branches except 'master'...
FOR /F "tokens=*" %%B IN ('git branch ^| findstr /V "master"') DO (
    echo Deleting local branch: %%B
    git branch -D %%B
)

echo.
echo Cleaning up remote branches except 'origin/master'...
git fetch --prune

FOR /F "tokens=*" %%R IN ('git branch -r ^| findstr /V "origin/master"') DO (
    SETLOCAL ENABLEDELAYEDEXPANSION
    SET "BRANCH=%%R"
    SET "BRANCH=!BRANCH:origin/=!"
    echo Deleting remote branch: !BRANCH!
    git push origin --delete !BRANCH!
    ENDLOCAL
)

echo.
echo Branch cleanup complete.
pause
