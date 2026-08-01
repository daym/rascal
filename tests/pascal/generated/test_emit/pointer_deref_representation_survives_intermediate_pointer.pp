unit u;
interface
type plongint = ^longint;
procedure run(raw : pointer; value : longint; var result : longint);
implementation
procedure run(raw : pointer; value : longint; var result : longint);
var p : plongint;
begin
  plongint(raw)^ := value;
  result := plongint(raw)^;
  p := plongint(raw);
  p^ := value;
  result := p^;
end;
end.
