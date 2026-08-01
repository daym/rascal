unit u;
interface
type
  plongint = ^longint;
  tbase = class end;
  tchild = class(tbase)
    value : longint;
  end;
function field_addr(p : tbase) : plongint;
implementation
function field_addr(p : tbase) : plongint;
begin
  field_addr := @tchild(p).value;
end;
end.
