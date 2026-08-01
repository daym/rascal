unit u;
interface
type
  tbuf = array[0..3] of char;
const
  buf : tbuf = 'abc';
  text : pchar = @buf;
implementation
end.
