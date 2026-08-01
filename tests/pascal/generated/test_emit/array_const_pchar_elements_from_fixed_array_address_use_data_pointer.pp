unit u;
interface
type
  tbuf = array[0..3] of char;
  ttexts = array[0..1] of pchar;
const
  buf : tbuf = 'abc';
  texts : ttexts = (@buf, @buf);
implementation
end.
