unit u;
interface
type
  tsmall = set of 0..7;
  tbytes = set of byte;
procedure take(xs : tbytes); overload;
procedure take(n : longint); overload;
procedure run(s : tsmall);
implementation
procedure take(xs : tbytes); begin end;
procedure take(n : longint); begin end;
procedure run(s : tsmall);
begin
  take(s);
end;
end.
