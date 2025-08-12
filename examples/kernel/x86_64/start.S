
.global _start
_start:
  cli
  call kmain
1: hlt
   jmp 1b
