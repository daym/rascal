unit u;
interface
type
  tai = record end;
  tasmop = (a_add, a_sub, a_mul);
  topsize = (s_no, s_l);
  topsizes = set of topsize;
function match(instr : tai; op : tasmop; sizes : topsizes) : boolean; overload;
function match(instr : tai; const ops : array of tasmop; sizes : topsizes) : boolean; overload;
procedure demo(instr : tai);
implementation
function match(instr : tai; op : tasmop; sizes : topsizes) : boolean;
begin
  match := true;
end;
function match(instr : tai; const ops : array of tasmop; sizes : topsizes) : boolean;
begin
  match := true;
end;
procedure demo(instr : tai);
begin
  if match(instr, [a_add, a_sub], [s_no]) then begin end;
end;
end.
