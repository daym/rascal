unit u;
interface
type
  tqueue = procedure(msg : integer) of object;
procedure note(onqueue : tqueue = nil);
procedure run;
implementation
procedure note(onqueue : tqueue);
begin
end;
procedure run;
begin
  note;
end;
end.
