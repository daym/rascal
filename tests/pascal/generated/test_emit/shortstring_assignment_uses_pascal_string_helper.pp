unit u;
interface
type
  pstring = ^string;
procedure store_string(const s : string; var p : pstring);
implementation
procedure store_string(const s : string; var p : pstring);
begin
  getmem(p, length(s) + 1);
  p^ := s;
end;
end.
