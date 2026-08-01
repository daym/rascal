unit u;
interface
procedure demo(s : string; var i : longint);
implementation
procedure demo(s : string; var i : longint);
begin
  case s of
    'one' : i := 1;
    'two', 'dos' : i := 2;
  else
    i := 0;
  end;
end;
end.
