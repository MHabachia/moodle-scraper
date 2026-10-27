import {describe, expect, it} from 'vitest';
import cargoToml from '../src-tauri/Cargo.toml?raw';
import changelog from '../CHANGELOG.md?raw';
import packageJson from '../package.json?raw';
import tauriConf from '../src-tauri/tauri.conf.json?raw';

/** Die Version steht an mehreren Stellen; ein Release mit Abweichungen soll gar nicht erst gebaut werden. */
describe('Versionsnummern', () => {
  const pkg = JSON.parse(packageJson).version as string;

  it('stimmen in allen Projektdateien überein', () => {
    expect(JSON.parse(tauriConf).version).toBe(pkg);
    expect(/^version = "([^"]+)"/m.exec(cargoToml)?.[1]).toBe(pkg);
  });

  it('stehen als oberster Eintrag im Changelog', () => {
    expect(/^## (\d+\.\d+\.\d+)/m.exec(changelog)?.[1]).toBe(pkg);
  });
});
