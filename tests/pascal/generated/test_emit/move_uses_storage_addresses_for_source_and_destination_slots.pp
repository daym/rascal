unit u;
interface
procedure slide(index, count : longint);
implementation
procedure slide(index, count : longint);
var
  list : array[0..3] of pointer;
begin
  system.move(list[index + 1], list[index],
              (count - index) * sizeof(pointer));
end;
end.
