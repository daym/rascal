unit u;
interface
type
  trec = record
    name : pshortstring;
    data : ppointer;
    case longint of
      0: (alt : pshortstring);
  end;
procedure take(var p : pshortstring; q : ppointer);
implementation
procedure take(var p : pshortstring; q : ppointer);
begin
end;
end.
