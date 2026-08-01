unit u;
interface
type
  tflag = (fa, fb, fc);
  tflags = set of tflag;
  tsmall = set of 2..5;
const
  lastflag = ord(high(tflags));
  firstsmall = low(tsmall);
  lastsmall = high(tsmall);
procedure run;
implementation
procedure run;
begin
  if ord(high(tflags)) > 31 then begin end;
end;
end.
