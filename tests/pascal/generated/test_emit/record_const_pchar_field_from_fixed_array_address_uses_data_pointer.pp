unit u;
interface
type
  tbuf = array[0..3] of char;
  trec = record
    text : pchar;
  end;
const
  buf : tbuf = 'abc';
  rec : trec = (text: @buf);
implementation
end.
