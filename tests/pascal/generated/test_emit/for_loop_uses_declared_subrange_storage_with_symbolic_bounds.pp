unit u;
interface
function swapbytes(d : double) : double;
implementation
function swapbytes(d : double) : double;
type
  tbytes = array[0..sizeof(d)-1] of byte;
var
  i : 0..sizeof(d)-1;
begin
  for i := low(tbytes) to high(tbytes) do
    tbytes(swapbytes)[i] := tbytes(d)[high(tbytes)-i];
end;
end.
