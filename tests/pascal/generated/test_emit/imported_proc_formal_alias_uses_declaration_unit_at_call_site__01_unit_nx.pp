unit nx;
interface
uses globals, cpuinfo;
procedure use;
implementation
procedure use;
var value_real : bestreal;
begin
  get_real_sign(value_real);
end;
end.
