unit u;
interface
type
  tstream = object
    function write(const buffer; count : longint) : longint;
  end;
implementation
function tstream.write(const buffer; count : longint) : longint;
begin
end;
end.
