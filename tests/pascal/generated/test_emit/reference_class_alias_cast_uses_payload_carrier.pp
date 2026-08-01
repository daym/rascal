unit u;
interface
type
  tbase = class end;
  tchild = class(tbase)
    ops : longint;
  end;
  tinstr = tchild;
  pinstr = ^tinstr;
function read_alias(p : tbase) : longint;
function read_pointer_alias(p : tbase) : longint;
implementation
function read_alias(p : tbase) : longint;
begin
  read_alias := tinstr(p).ops;
end;
function read_pointer_alias(p : tbase) : longint;
begin
  read_pointer_alias := pinstr(p)^.ops;
end;
end.
