unit u;
interface
type
  titem = object
  end;
  pitem = ^titem;
  trec = packed record
    item : pitem;
  end;
procedure take(var p : pitem);
procedure run(var r : trec);
implementation
procedure take(var p : pitem);
begin
end;
procedure run(var r : trec);
begin
  take(r.item);
end;
end.
