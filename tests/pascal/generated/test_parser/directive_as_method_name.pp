unit u;
interface
type
  tcoll = object
    function at(i : integer) : longint;
  end;
implementation
function tcoll.at(i : integer) : longint;
begin
  at := i;
end;
end.
