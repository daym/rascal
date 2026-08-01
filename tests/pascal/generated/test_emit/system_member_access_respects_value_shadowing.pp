unit u;
interface
type
  trec = record
    heapsize : longint;
  end;
procedure run(system : trec; var n : longint);
implementation
procedure run(system : trec; var n : longint);
begin
  n := system.heapsize;
end;
end.
