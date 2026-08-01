unit u;
interface
type
  TSys = (s1, s2);
  TSystems = set of TSys;
const
  ControllerSupport = true;
  SystemsEmbedded : TSystems = [s1];
procedure demo(system : TSys; c : longint; s : string);
implementation
procedure demo(system : TSys; c : longint; s : string);
begin
  if ControllerSupport and (system in SystemsEmbedded) and
     (c <> 0) and (s <> '') then writeln(c);
end;
end.
