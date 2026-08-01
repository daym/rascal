unit u;
interface
type
  tcode = array[0..3] of char;
  pcode = ^tcode;
  trec = record
    code : tcode;
  end;
  prec = ^trec;
procedure demo(p : prec; var pc : pchar; var pa : pcode);
implementation
procedure demo(p : prec; var pc : pchar; var pa : pcode);
begin
  pc := @p^.code;
  pa := @p^.code;
end;
end.
