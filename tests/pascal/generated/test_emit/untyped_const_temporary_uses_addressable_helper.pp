unit u;
interface
type
  trec = record
    a, b : longint;
  end;
  twriter = object
    procedure sink(const b; len : longint);
  end;
function buildrec(x : longint) : trec;
procedure demo;
implementation
procedure twriter.sink(const b; len : longint);
begin
end;
function buildrec(x : longint) : trec;
begin
end;
procedure demo;
var
  w : twriter;
begin
  w.sink(buildrec(42), sizeof(trec));
end;
end.
