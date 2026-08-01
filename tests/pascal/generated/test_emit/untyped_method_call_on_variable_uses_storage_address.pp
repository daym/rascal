unit u;
interface
type
  tstream = object
    function read(var buffer; count : longint) : longint;
    function copyfrom(source : tstream; count : longint) : longint;
  end;
implementation
function tstream.read(var buffer; count : longint) : longint;
begin
end;
function tstream.copyfrom(source : tstream; count : longint) : longint;
var
  buffer : array[0..3] of byte;
begin
  copyfrom := source.read(buffer, count);
end;
end.
