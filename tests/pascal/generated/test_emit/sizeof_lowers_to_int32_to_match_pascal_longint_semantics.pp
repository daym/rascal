unit u;
interface
type myrec = record x : longint; end;
function note(n : longint) : longint;
implementation
function note(n : longint) : longint;
begin
  note := sizeof(myrec);
end;
end.
