unit u;
interface
procedure zero(capacity : longint);
implementation
procedure zero(capacity : longint);
var
  list : array[0..3] of pointer;
begin
  fillchar(list[capacity], (4 - capacity) * sizeof(pointer), 0);
end;
end.
