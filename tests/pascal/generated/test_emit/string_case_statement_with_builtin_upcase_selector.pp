unit u;
interface
procedure demo(s : string; var i : longint);
implementation
procedure demo(s : string; var i : longint);
begin
  case UpCase(s) of
    'CS': i := 1;
    'DS', 'ES': i := 2;
  else
    i := 0;
  end;
end;
end.
