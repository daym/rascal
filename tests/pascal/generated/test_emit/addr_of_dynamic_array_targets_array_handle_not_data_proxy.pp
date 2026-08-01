unit u;
interface
type
  tbytes = array of byte;
  pbytes = ^tbytes;
  trec = record
    use : tbytes;
  end;
  prec = ^trec;
procedure demo(var a : tbytes; r : prec; var pa, pr : pbytes);
implementation
procedure demo(var a : tbytes; r : prec; var pa, pr : pbytes);
begin
  pa := @a;
  pr := @r^.use;
end;
end.
