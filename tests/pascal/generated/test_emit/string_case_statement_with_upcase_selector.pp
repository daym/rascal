unit u;
interface
procedure demo(s : string; var i : longint);
implementation
procedure demo(s : string; var i : longint);
function to_upper(s: string): string;
begin
  to_upper := s;
end;
begin
  case to_upper(s) of
    'CS': i := 1;
    'DS', 'ES': i := 2;
  else
    i := 0;
  end;
end;
end.
