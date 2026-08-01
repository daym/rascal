unit globals;
interface
uses cpuinfo;
function get_real_sign(r : bestreal) : longint;
implementation
function get_real_sign(r : bestreal) : longint;
begin
  get_real_sign := 0;
end;
end.
