unit u;
interface
type
  tbase = class
    destructor destroy; override;
  end;
  tmid = class(tbase)
  end;
  tleaf = class(tmid)
    destructor destroy; override;
  end;
implementation
destructor tbase.destroy;
begin
end;
destructor tleaf.destroy;
begin
  inherited destroy;
end;
end.
