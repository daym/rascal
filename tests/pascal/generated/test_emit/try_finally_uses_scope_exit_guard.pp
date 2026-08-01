unit u;
interface
procedure demo;
implementation
var x : integer;
procedure demo;
begin
  try
    x := 1;
  finally
    x := 2;
  end;
end;
end.
