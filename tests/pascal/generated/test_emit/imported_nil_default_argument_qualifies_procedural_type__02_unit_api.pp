unit api;
interface
type
  tcallback = procedure(v : longint) of object;
procedure note(cb : tcallback = nil);
implementation
procedure note(cb : tcallback);
begin
end;
end.
