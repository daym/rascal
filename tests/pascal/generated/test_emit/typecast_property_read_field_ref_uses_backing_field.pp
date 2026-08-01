unit u;
interface
type
  tslot = record
    value : longint;
  end;
  tbase = class end;
  tbox = class(tbase)
  private
    fslot : tslot;
  public
    property slot : tslot read fslot;
  end;
procedure take(const s : tslot);
procedure run(b : tbase);
implementation
procedure take(const s : tslot);
begin
end;
procedure run(b : tbase);
begin
  take(tbox(b).slot);
end;
end.
