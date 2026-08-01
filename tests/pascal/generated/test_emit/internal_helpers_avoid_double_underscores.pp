unit u;
interface
type
  tcolor = (red, green);
  tcb = procedure(x : integer) of object;
  tobj = object
    cb : tcb;
    procedure fire(x : integer);
    procedure run(var i : integer);
  end;
implementation
procedure tobj.fire(x : integer);
begin
end;
procedure tobj.run(var i : integer);
begin
  with self do cb := fire;
  for i := low(tcolor) to high(tcolor) do begin end;
end;
end.
