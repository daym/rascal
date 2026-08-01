unit u;
interface
procedure demo(s : string; var i : longint);
implementation
procedure demo(s : string; var i : longint);
begin
  case s of
    'a' : i := 1;
  else
    i := 0;
  end;
end;
end.
