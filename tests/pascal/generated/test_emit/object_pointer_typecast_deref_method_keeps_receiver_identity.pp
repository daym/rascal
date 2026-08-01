unit u;
interface
type
  tobj = object
    value : longint;
    procedure reset;
  end;
  pobj = ^tobj;
procedure clear(p : pointer);
implementation
procedure tobj.reset;
begin
  value := 0;
end;
procedure clear(p : pointer);
begin
  pobj(p)^.reset;
end;
end.
