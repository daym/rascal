unit u;
interface
procedure run(var s : ansistring; i : longint; c : char);
implementation
procedure run(var s : ansistring; i : longint; c : char);
begin
  c := s[i + 1];
  s[i + 1] := c;
end;
end.
