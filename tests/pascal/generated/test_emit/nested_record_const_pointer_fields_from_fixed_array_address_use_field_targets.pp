unit u;
interface
type
  tbuf = array[0..3] of char;
  pbuf = ^tbuf;
  trec = record
    text : pchar;
    data : pbuf;
  end;
const
  buf : tbuf = 'abc';
  items : array[0..0] of trec = ((text: @buf; data: @buf));
implementation
end.
