unit dep;
interface
type
  tbase = object end;
  tchild = object(tbase) end;
  pchild = ^tchild;
procedure take(p : ^tbase);
implementation
procedure take(p : ^tbase);
begin
end;
end.
