unit u;
interface
type
  tmsg = object
    function get(nr : integer; const args : array of string) : string;
  end;
implementation
function tmsg.get(nr : integer; const args : array of string) : string;
begin
end;
end.
