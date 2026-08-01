unit u;
interface
type
  tbuf = array[0..3] of char;
  pbuf = ^tbuf;
  trec = record
    data : pbuf;
  end;
const
  buf : tbuf = 'abc';
  rec : trec = (data: @buf);
implementation
end.
