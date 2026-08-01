unit u;
interface
type
  trec = record
    x : longint;
  end;
procedure take(constref r : trec);
function make : trec;
implementation
procedure take(constref r : trec);
begin
  if r.x <> 0 then ;
end;
function make : trec;
begin
  make.x := 1;
end;
procedure demo;
begin
  take(make);
end;
end.
