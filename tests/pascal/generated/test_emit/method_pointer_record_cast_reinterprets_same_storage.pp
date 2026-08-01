unit u;
interface
type
  tcb = procedure(x : integer) of object;
  trec = record
    procpointer : pointer;
    s : pointer;
  end;
procedure setslots(var p : tcb; addr : pointer; selfp : pointer);
implementation
procedure setslots(var p : tcb; addr : pointer; selfp : pointer);
begin
  trec(p).procpointer := addr;
  trec(p).s := selfp;
end;
end.
