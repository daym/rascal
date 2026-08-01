unit ops;
interface
type
  trec = record
    svalue : int64;
  end;
operator := (const s : int64) : trec;
operator <= (const a, b : trec) : boolean;
procedure test;
implementation
operator := (const s : int64) : trec;
begin
  result.svalue := s;
end;
operator <= (const a, b : trec) : boolean;
begin
  result := a.svalue <= b.svalue;
end;
procedure test;
var r : trec; cond : boolean;
begin
  cond := r <= system.low(int64) div 2;
end;
end.
