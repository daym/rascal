unit u;
interface
procedure narrow_r(v : extended);
procedure narrow_n(v : extended);
implementation
{$R+}
procedure narrow_r(v : extended);
var c : int64;
begin
  c := v;
end;
{$R-}
procedure narrow_n(v : extended);
var c : int64;
begin
  c := v;
end;
end.
