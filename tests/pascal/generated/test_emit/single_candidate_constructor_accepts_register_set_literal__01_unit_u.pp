unit u;
interface
uses rgcpu, cgbase, cpuinfo;
procedure run;
implementation
procedure run;
var rg : trgcpu;
begin
  rg := trgcpu.create(r_intregister, sub_whole,
    [rs_eax, rs_edx], $10, [rs_ebp]);
  rg.alloccpuregisters(r_intregister, [rs_function_result_reg]);
end;
end.
