unit u;
interface
function distance(stored : pointer; current : ptrint) : ptruint;
implementation
function distance(stored : pointer; current : ptrint) : ptruint;
begin
  distance := ptruint(abs(ptrint(stored) - current));
end;
end.
