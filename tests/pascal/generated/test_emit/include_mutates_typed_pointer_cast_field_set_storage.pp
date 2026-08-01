unit u;
interface
type
  tregister = (r0, r1);
  tregset = set of tregister;
  ttaiprop = record
    usedregs : tregset;
  end;
  ptaiprop = ^ttaiprop;
  tai = object
    optinfo : pointer;
  end;
  pai = ^tai;
procedure mark(p : pai; reg : tregister);
implementation
procedure mark(p : pai; reg : tregister);
begin
  include(ptaiprop(p^.optinfo)^.usedregs, reg);
end;
end.
