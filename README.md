# runfromprocess-rs

I want to launch `myapp.exe --my_arg` from `mylauncher.exe`

1. Find parent process id

   ```PowerShell
   tasklist /fo list /fi "IMAGENAME eq mylauncher.exe"
   ```

2. Launch the child process using the parent process id

   ```PowerShell
   runfromprocess-rs.exe $parent_process_id myapp.exe --my_arg
   ```
