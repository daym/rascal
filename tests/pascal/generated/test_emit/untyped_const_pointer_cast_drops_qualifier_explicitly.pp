unit u;
interface
type
  tobj = object
    procedure load(const b);
  end;
implementation
procedure tobj.load(const b);
var
  p : pchar;
begin
  p := pchar(@b);
end;
end.
