unit u;
interface
type
  tobj = object
    procedure fire(x : integer);
  end;
const
  addr : pointer = @tobj.fire;
implementation
procedure tobj.fire(x : integer);
begin
end;
end.
