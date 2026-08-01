unit globals;
interface
uses linux;
function FixPath(s : string; allowdot : boolean) : string;
implementation
function FixPath(s : string; allowdot : boolean) : string;
begin
  FixPath := s;
end;
end.
