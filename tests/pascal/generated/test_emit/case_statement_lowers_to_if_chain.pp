unit u;
interface
procedure demo(var i : longint);
implementation
procedure demo(var i : longint);
begin
  case i of
    1 : i := 10;
    2, 3 : i := 20;
    4..6 : i := 30;
  else
    i := 99;
  end;
end;
end.
