unit u;
interface
type bestreal = extended;
function pick(i : longint) : longint; overload;
function pick(r : bestreal) : longint; overload;
procedure run(i : longint; var out_i : longint);
implementation
function pick(i : longint) : longint; begin pick := 1; end;
function pick(r : bestreal) : longint; begin pick := 2; end;
procedure run(i : longint; var out_i : longint);
begin
  out_i := pick(i);
end;
end.
