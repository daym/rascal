unit u;
interface
type
  tkey = (none);
  trec = record
    name : string[20];
    size : longint;
  end;
const
  items : array[tkey] of trec = ((name:''; size:0));
implementation
end.
