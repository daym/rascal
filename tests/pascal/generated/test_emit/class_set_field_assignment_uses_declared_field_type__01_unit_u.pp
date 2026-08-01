unit u;
interface
uses nbas, cpubase;
procedure run;
implementation
procedure run;
var asmstat : tasmnode;
begin
  asmstat.used_regs_fpu := [0..first_fpu_imreg-1];
end;
end.
