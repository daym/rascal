unit u;
interface
uses dos;
function fixpath(s : string; allowdot : boolean) : string;
procedure demo;
implementation
function fixpath(s : string; allowdot : boolean) : string;
begin
  fixpath := s;
end;
procedure demo;
var p : string;
begin
  p := fixpath(fexpand(p), false);
end;
end.
