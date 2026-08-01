unit u;
interface
const
  RS_CR0 = 1;
  RS_CR7 = 8;
type
  TCR = RS_CR0..RS_CR7;
  TRec = record
    cr: TCR;
  end;
procedure take(v: longint);
procedure run;
implementation
procedure take(v: longint);
begin
end;
procedure run;
var r: TRec;
begin
  take(((r.cr - RS_CR0) * 4 + 3) and 31);
end;
end.
