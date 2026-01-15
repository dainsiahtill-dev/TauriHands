<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import * as THREE from "three";
import { EffectComposer } from "three/examples/jsm/postprocessing/EffectComposer.js";
import { RenderPass } from "three/examples/jsm/postprocessing/RenderPass.js";
import { UnrealBloomPass } from "three/examples/jsm/postprocessing/UnrealBloomPass.js";
import { OutputPass } from "three/examples/jsm/postprocessing/OutputPass.js";

type ScreenNode = {
  group: THREE.Group;
  canvas: HTMLCanvasElement;
  ctx: CanvasRenderingContext2D;
  texture: THREE.CanvasTexture;
  text: string;
  title: string;
  lastUpdate: number;
};

const containerRef = ref<HTMLDivElement | null>(null);
let cleanupScene: (() => void) | null = null;
let composer: EffectComposer | null = null;
let bloomPass: UnrealBloomPass | null = null;

onMounted(() => {
  const container = containerRef.value;
  if (!container) return;

  const prefersReducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
  const root = container.closest(".app-shell") as HTMLElement | null;

  const scene = new THREE.Scene();
  scene.background = new THREE.Color(0x00050a);
  scene.fog = new THREE.FogExp2(0x00050a, 0.035);

  const hudScene = new THREE.Scene();

  const camera = new THREE.PerspectiveCamera(50, 1, 0.1, 200);
  camera.position.set(0, 0.6, 8.4);
  camera.lookAt(0, 0.2, 0);

  const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
  renderer.setSize(container.clientWidth, container.clientHeight);
  renderer.setClearColor(0x000000, 0);
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1.08;
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.autoClear = false;
  renderer.domElement.className = "cyber-scene__canvas";
  container.appendChild(renderer.domElement);

  const renderPass = new RenderPass(scene, camera);
  const hudPass = new RenderPass(hudScene, camera);
  hudPass.clear = false;
  hudPass.clearDepth = true;

  bloomPass = new UnrealBloomPass(
    new THREE.Vector2(container.clientWidth, container.clientHeight),
    1.1,
    0.6,
    0.1,
  );

  const outputPass = new OutputPass();

  composer = new EffectComposer(renderer);
  composer.addPass(renderPass);
  composer.addPass(hudPass);
  composer.addPass(bloomPass);
  composer.addPass(outputPass);

  const ambient = new THREE.AmbientLight(0x0b1520, 0.25);
  const roomLight = new THREE.PointLight(0x0066ff, 1.4, 26);
  roomLight.position.set(0, 4.5, 2);
  const cyanLeft = new THREE.PointLight(0x00f3ff, 0.9, 18);
  cyanLeft.position.set(-4.2, 1.2, -4.2);
  const cyanRight = new THREE.PointLight(0x00f3ff, 0.9, 18);
  cyanRight.position.set(4.2, 1.2, -4.2);
  scene.add(ambient, roomLight, cyanLeft, cyanRight);

  const roomSize = 10;
  const halfSize = roomSize / 2;
  const roomObjects: THREE.Object3D[] = [];
  const screenNodes: ScreenNode[] = [];

  const wallMaterial = new THREE.MeshStandardMaterial({
    color: 0x050a12,
    roughness: 0.85,
    metalness: 0.25,
    side: THREE.DoubleSide,
  });

  const gridMajor = 0x004466;
  const gridMinor = 0x001122;

  function addGrid(size: number, divisions: number, position: THREE.Vector3, rotation: THREE.Euler) {
    const grid = new THREE.GridHelper(size, divisions, gridMajor, gridMinor);
    grid.position.copy(position);
    grid.rotation.copy(rotation);
    const materials = Array.isArray(grid.material) ? grid.material : [grid.material];
    materials.forEach((material, index) => {
      material.transparent = true;
      material.opacity = index === 0 ? 0.35 : 0.16;
    });
    scene.add(grid);
    roomObjects.push(grid);
  }

  function addWall(geometry: THREE.PlaneGeometry, position: THREE.Vector3, rotation: THREE.Euler) {
    const wall = new THREE.Mesh(geometry, wallMaterial);
    wall.position.copy(position);
    wall.rotation.copy(rotation);
    scene.add(wall);
    roomObjects.push(wall);
  }

  addWall(new THREE.PlaneGeometry(roomSize, roomSize), new THREE.Vector3(0, -halfSize, 0), new THREE.Euler(-Math.PI / 2, 0, 0));
  addGrid(
    roomSize,
    20,
    new THREE.Vector3(0, -halfSize + 0.02, 0),
    new THREE.Euler(0, 0, 0),
  );

  addWall(new THREE.PlaneGeometry(roomSize, roomSize), new THREE.Vector3(0, halfSize, 0), new THREE.Euler(Math.PI / 2, 0, 0));
  addGrid(
    roomSize,
    20,
    new THREE.Vector3(0, halfSize - 0.02, 0),
    new THREE.Euler(0, 0, 0),
  );

  addWall(new THREE.PlaneGeometry(roomSize, roomSize), new THREE.Vector3(0, 0, -halfSize), new THREE.Euler(0, 0, 0));
  addGrid(
    roomSize,
    20,
    new THREE.Vector3(0, 0, -halfSize + 0.02),
    new THREE.Euler(Math.PI / 2, 0, 0),
  );

  addWall(new THREE.PlaneGeometry(roomSize, roomSize), new THREE.Vector3(-halfSize, 0, 0), new THREE.Euler(0, Math.PI / 2, 0));
  addGrid(
    roomSize,
    20,
    new THREE.Vector3(-halfSize + 0.02, 0, 0),
    new THREE.Euler(0, 0, Math.PI / 2),
  );

  addWall(new THREE.PlaneGeometry(roomSize, roomSize), new THREE.Vector3(halfSize, 0, 0), new THREE.Euler(0, -Math.PI / 2, 0));
  addGrid(
    roomSize,
    20,
    new THREE.Vector3(halfSize - 0.02, 0, 0),
    new THREE.Euler(0, 0, Math.PI / 2),
  );

  const roomFrame = new THREE.LineSegments(
    new THREE.EdgesGeometry(new THREE.BoxGeometry(roomSize, roomSize, roomSize)),
    new THREE.LineBasicMaterial({ color: 0x00f3ff, transparent: true, opacity: 0.28 }),
  );
  scene.add(roomFrame);
  roomObjects.push(roomFrame);

  function makeCurvedPlane(width: number, height: number, radius: number) {
    const thetaLength = width / radius;
    const geometry = new THREE.CylinderGeometry(
      radius,
      radius,
      height,
      64,
      1,
      true,
      Math.PI + (Math.PI - thetaLength) / 2,
      thetaLength,
    );
    geometry.rotateZ(Math.PI);
    geometry.rotateY(Math.PI / 2);
    return geometry;
  }

  function drawScreen(node: ScreenNode, time: number) {
    const { ctx, canvas, text, title } = node;
    const width = canvas.width;
    const height = canvas.height;

    ctx.clearRect(0, 0, width, height);
    ctx.fillStyle = "rgba(0, 8, 20, 0.82)";
    ctx.fillRect(0, 0, width, height);

    ctx.strokeStyle = "rgba(0, 243, 255, 0.08)";
    ctx.lineWidth = 1;
    for (let x = 0; x <= width; x += 64) {
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, height);
      ctx.stroke();
    }
    for (let y = 0; y <= height; y += 64) {
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(width, y);
      ctx.stroke();
    }

    ctx.fillStyle = "rgba(0, 243, 255, 0.15)";
    ctx.fillRect(0, 0, width, 56);
    ctx.fillStyle = "#00f3ff";
    ctx.font = 'bold 24px "JetBrains Mono"';
    ctx.textAlign = "left";
    ctx.fillText(title, 32, 36);
    ctx.textAlign = "right";
    ctx.font = '16px "JetBrains Mono"';
    ctx.fillText("SECURE", width - 28, 34);
    ctx.textAlign = "left";

    ctx.fillStyle = "#c8f7ff";
    ctx.font = '22px "JetBrains Mono"';
    ctx.shadowColor = "rgba(0, 243, 255, 0.6)";
    ctx.shadowBlur = 6;
    const lines = text.split("\n");
    let y = 96;
    lines.forEach((line) => {
      ctx.fillText(line, 40, y);
      y += 40;
    });
    ctx.shadowBlur = 0;

    ctx.strokeStyle = "rgba(0, 243, 255, 0.35)";
    ctx.lineWidth = 2;
    ctx.strokeRect(8, 8, width - 16, height - 16);

    const scanY = ((time * 18) % (height + 80)) - 40;
    const gradient = ctx.createLinearGradient(0, scanY - 18, 0, scanY + 18);
    gradient.addColorStop(0, "rgba(0, 243, 255, 0)");
    gradient.addColorStop(0.5, "rgba(0, 243, 255, 0.08)");
    gradient.addColorStop(1, "rgba(0, 243, 255, 0)");
    ctx.fillStyle = gradient;
    ctx.fillRect(0, scanY - 18, width, 36);

    node.texture.needsUpdate = true;
  }

  function createCurvedScreen(options: {
    title: string;
    text: string;
    width?: number;
    height?: number;
    radius?: number;
    position: THREE.Vector3;
    rotation?: THREE.Euler;
    scale?: number;
  }) {
    const width = options.width ?? 3.6;
    const height = options.height ?? 2.1;
    const radius = options.radius ?? 10;

    const group = new THREE.Group();
    group.position.copy(options.position);
    if (options.rotation) {
      group.rotation.copy(options.rotation);
    }
    if (options.scale) {
      group.scale.setScalar(options.scale);
    }

    const canvas = document.createElement("canvas");
    canvas.width = 1024;
    canvas.height = 576;
    const ctx = canvas.getContext("2d");
    if (!ctx) return group;

    const texture = new THREE.CanvasTexture(canvas);
    texture.anisotropy = Math.min(8, renderer.capabilities.getMaxAnisotropy());
    texture.colorSpace = THREE.SRGBColorSpace;

    const geometry = makeCurvedPlane(width, height, radius);
    const screen = new THREE.Mesh(
      geometry,
      new THREE.MeshBasicMaterial({
        map: texture,
        transparent: true,
        opacity: 0.92,
        color: 0xffffff,
        side: THREE.DoubleSide,
        blending: THREE.AdditiveBlending,
        depthWrite: false,
      }),
    );
    group.add(screen);

    const glass = new THREE.Mesh(
      geometry,
      new THREE.MeshPhysicalMaterial({
        color: 0x99f6ff,
        metalness: 0.1,
        roughness: 0.1,
        transmission: 0.9,
        transparent: true,
        opacity: 0.22,
        side: THREE.DoubleSide,
      }),
    );
    glass.scale.set(1.01, 1.01, 1.01);
    group.add(glass);

    const frameGeometry = makeCurvedPlane(width + 0.12, 0.12, radius);
    const frameMaterial = new THREE.MeshStandardMaterial({
      color: 0x001122,
      roughness: 0.35,
      metalness: 0.85,
      emissive: 0x002244,
      emissiveIntensity: 0.6,
    });
    const topFrame = new THREE.Mesh(frameGeometry, frameMaterial);
    topFrame.position.y = height / 2 + 0.07;
    group.add(topFrame);

    const bottomFrame = new THREE.Mesh(frameGeometry, frameMaterial);
    bottomFrame.position.y = -height / 2 - 0.07;
    group.add(bottomFrame);

    scene.add(group);
    screenNodes.push({
      group,
      canvas,
      ctx,
      texture,
      text: options.text,
      title: options.title,
      lastUpdate: 0,
    });

    return group;
  }

  createCurvedScreen({
    title: "COMMAND_CENTER",
    text: "Status: ONLINE\nDefenses: ACTIVE\nNodes: 142",
    position: new THREE.Vector3(0, 1.4, -4.4),
    scale: 1.5,
  });

  createCurvedScreen({
    title: "SERVER_LIST",
    text: "Alpha: OK\nBeta: OK\nGamma: WARN\nDelta: OK",
    position: new THREE.Vector3(-4.4, 1.4, 0),
    rotation: new THREE.Euler(0, Math.PI / 2, 0),
  });

  createCurvedScreen({
    title: "INTEL_MAP",
    text: "Sector 7: Clear\nSector 8: Activity\nDrone 1: Return",
    position: new THREE.Vector3(4.4, 1.4, 0),
    rotation: new THREE.Euler(0, -Math.PI / 2, 0),
  });

  createCurvedScreen({
    title: "TOPOGRAPHY_SCAN",
    text: "Terrain: Flat\nGrid: Locked\nSync: 98%",
    position: new THREE.Vector3(0, -4.7, 0),
    rotation: new THREE.Euler(-Math.PI / 2, 0, 0),
    radius: 100,
    width: 3.8,
    height: 2.2,
  });

  screenNodes.forEach((node) => drawScreen(node, 0));

  const hudSceneGroup = new THREE.Group();
  hudScene.add(hudSceneGroup);

  const hudMaterials = {
    frame: new THREE.MeshBasicMaterial({
      color: 0x00f3ff,
      transparent: true,
      opacity: 0.32,
      blending: THREE.AdditiveBlending,
    }),
    accent: new THREE.MeshBasicMaterial({
      color: 0x0088ff,
      transparent: true,
      opacity: 0.26,
      blending: THREE.AdditiveBlending,
    }),
    glass: new THREE.MeshBasicMaterial({
      color: 0x001020,
      transparent: true,
      opacity: 0.2,
    }),
    line: new THREE.LineBasicMaterial({
      color: 0x00f3ff,
      transparent: true,
      opacity: 0.45,
      blending: THREE.AdditiveBlending,
    }),
    lineAccent: new THREE.LineBasicMaterial({
      color: 0x0088ff,
      transparent: true,
      opacity: 0.38,
      blending: THREE.AdditiveBlending,
    }),
  };

  const hudGeometries: THREE.BufferGeometry[] = [];
  const hudDistance = 10;

  function trackGeometry(geometry: THREE.BufferGeometry) {
    hudGeometries.push(geometry);
    return geometry;
  }

  function clearHud() {
    hudSceneGroup.clear();
    hudGeometries.forEach((geometry) => geometry.dispose());
    hudGeometries.length = 0;
  }

  function buildHud() {
    clearHud();

    const height = 2 * Math.tan(THREE.MathUtils.degToRad(camera.fov * 0.5)) * hudDistance;
    const width = height * camera.aspect;
    const frameWidth = width * 0.96;
    const frameHeight = height * 0.9;
    const barThickness = height * 0.045;
    const barDepth = 0.55;
    const innerWidth = frameWidth * 0.88;
    const innerHeight = frameHeight * 0.72;

    const topBar = new THREE.Mesh(
      trackGeometry(new THREE.BoxGeometry(frameWidth, barThickness, barDepth)),
      hudMaterials.frame,
    );
    topBar.position.set(0, frameHeight / 2 - barThickness / 2, 0);
    hudSceneGroup.add(topBar);

    const bottomBar = new THREE.Mesh(
      trackGeometry(new THREE.BoxGeometry(frameWidth, barThickness, barDepth)),
      hudMaterials.frame,
    );
    bottomBar.position.set(0, -frameHeight / 2 + barThickness / 2, 0);
    hudSceneGroup.add(bottomBar);

    const sideHeight = frameHeight - barThickness * 1.1;
    const leftBar = new THREE.Mesh(
      trackGeometry(new THREE.BoxGeometry(barThickness, sideHeight, barDepth)),
      hudMaterials.frame,
    );
    leftBar.position.set(-frameWidth / 2 + barThickness / 2, 0, 0);
    hudSceneGroup.add(leftBar);

    const rightBar = new THREE.Mesh(
      trackGeometry(new THREE.BoxGeometry(barThickness, sideHeight, barDepth)),
      hudMaterials.frame,
    );
    rightBar.position.set(frameWidth / 2 - barThickness / 2, 0, 0);
    hudSceneGroup.add(rightBar);

    const cornerSize = barThickness * 1.3;
    const cornerDepth = barDepth * 1.4;
    const cornerPositions = [
      [-frameWidth / 2 + cornerSize / 2, frameHeight / 2 - cornerSize / 2],
      [frameWidth / 2 - cornerSize / 2, frameHeight / 2 - cornerSize / 2],
      [-frameWidth / 2 + cornerSize / 2, -frameHeight / 2 + cornerSize / 2],
      [frameWidth / 2 - cornerSize / 2, -frameHeight / 2 + cornerSize / 2],
    ];
    cornerPositions.forEach(([x, y]) => {
      const corner = new THREE.Mesh(
        trackGeometry(new THREE.BoxGeometry(cornerSize, cornerSize, cornerDepth)),
        hudMaterials.accent,
      );
      corner.position.set(x, y, 0.05);
      hudSceneGroup.add(corner);
    });

    const innerFrame = new THREE.LineSegments(
      trackGeometry(new THREE.EdgesGeometry(new THREE.PlaneGeometry(innerWidth, innerHeight))),
      hudMaterials.line,
    );
    innerFrame.position.set(0, 0, -0.25);
    hudSceneGroup.add(innerFrame);

    const topStrip = new THREE.Mesh(
      trackGeometry(new THREE.BoxGeometry(innerWidth * 0.7, barThickness * 0.35, barDepth * 0.6)),
      hudMaterials.accent,
    );
    topStrip.position.set(0, frameHeight / 2 - barThickness * 1.4, 0.05);
    hudSceneGroup.add(topStrip);

    const bottomDeck = new THREE.Mesh(
      trackGeometry(new THREE.BoxGeometry(innerWidth * 0.78, barThickness * 2.6, barDepth * 3.4)),
      hudMaterials.glass,
    );
    bottomDeck.position.set(0, -frameHeight / 2 + barThickness * 1.6, -barDepth * 1.6);
    bottomDeck.rotation.x = -0.25;
    hudSceneGroup.add(bottomDeck);

    const bottomDeckEdge = new THREE.LineSegments(
      trackGeometry(
        new THREE.EdgesGeometry(new THREE.BoxGeometry(innerWidth * 0.8, barThickness * 2.7, barDepth * 2.2)),
      ),
      hudMaterials.lineAccent,
    );
    bottomDeckEdge.position.copy(bottomDeck.position);
    bottomDeckEdge.rotation.copy(bottomDeck.rotation);
    hudSceneGroup.add(bottomDeckEdge);

    const innerAccent = new THREE.LineSegments(
      trackGeometry(new THREE.EdgesGeometry(new THREE.PlaneGeometry(innerWidth * 0.7, innerHeight * 0.6))),
      hudMaterials.lineAccent,
    );
    innerAccent.position.set(0, 0, -0.4);
    hudSceneGroup.add(innerAccent);

    const sideFinHeight = innerHeight * 0.45;
    const sideFin = new THREE.Mesh(
      trackGeometry(new THREE.BoxGeometry(barThickness * 1.1, sideFinHeight, barDepth * 0.8)),
      hudMaterials.accent,
    );
    sideFin.position.set(-frameWidth / 2 + barThickness * 0.8, -innerHeight * 0.1, 0.2);
    hudSceneGroup.add(sideFin);

    const sideFinRight = sideFin.clone();
    sideFinRight.position.set(frameWidth / 2 - barThickness * 0.8, -innerHeight * 0.1, 0.2);
    hudSceneGroup.add(sideFinRight);
  }

  const pointer = new THREE.Vector2(0, 0);
  const smoothed = new THREE.Vector2(0, 0);
  const cameraDirection = new THREE.Vector3();
  const clock = new THREE.Clock();
  const baseCamera = new THREE.Vector3(0, 0.6, 8.4);
  const lookTarget = new THREE.Vector3(0, 0.2, -1.5);
  let frameId = 0;

  function handlePointerMove(event: PointerEvent) {
    const bounds = container.getBoundingClientRect();
    const x = (event.clientX - bounds.left) / bounds.width - 0.5;
    const y = (event.clientY - bounds.top) / bounds.height - 0.5;
    pointer.set(x, y);
  }

  function handleResize() {
    const width = container.clientWidth;
    const height = container.clientHeight;
    camera.aspect = width / height;
    camera.updateProjectionMatrix();
    renderer.setSize(width, height);
    composer?.setSize(width, height);
    bloomPass?.setSize(width, height);
    buildHud();
  }

  function updateScene() {
    clock.getDelta();
    const elapsed = clock.elapsedTime;
    smoothed.lerp(pointer, 0.04);

    camera.position.x = THREE.MathUtils.lerp(camera.position.x, baseCamera.x + smoothed.x * 1.4, 0.05);
    camera.position.y = THREE.MathUtils.lerp(camera.position.y, baseCamera.y + smoothed.y * 0.9, 0.05);
    camera.position.z = THREE.MathUtils.lerp(camera.position.z, baseCamera.z, 0.05);
    camera.lookAt(lookTarget);

    camera.getWorldDirection(cameraDirection);
    hudSceneGroup.position.copy(camera.position).addScaledVector(cameraDirection, hudDistance);
    hudSceneGroup.quaternion.copy(camera.quaternion);

    screenNodes.forEach((node) => {
      if (elapsed - node.lastUpdate < 0.12) return;
      node.lastUpdate = elapsed;
      drawScreen(node, elapsed);
    });

    if (root) {
      root.style.setProperty("--scene-tilt-x", `${-smoothed.y * 1.6}deg`);
      root.style.setProperty("--scene-tilt-y", `${smoothed.x * 2}deg`);
    }

    if (composer) {
      composer.render();
    } else {
      renderer.clear();
      renderer.render(scene, camera);
      renderer.clearDepth();
      renderer.render(hudScene, camera);
    }
    frameId = window.requestAnimationFrame(updateScene);
  }

  window.addEventListener("pointermove", handlePointerMove, { passive: true });
  window.addEventListener("resize", handleResize);
  handleResize();

  if (!prefersReducedMotion.matches) {
    updateScene();
  } else {
    if (composer) {
      composer.render();
    } else {
      renderer.clear();
      renderer.render(scene, camera);
      renderer.clearDepth();
      renderer.render(hudScene, camera);
    }
  }

  cleanupScene = () => {
    window.removeEventListener("pointermove", handlePointerMove);
    window.removeEventListener("resize", handleResize);
    if (frameId) window.cancelAnimationFrame(frameId);
    if (root) {
      root.style.setProperty("--scene-tilt-x", "0deg");
      root.style.setProperty("--scene-tilt-y", "0deg");
    }
    clearHud();
    screenNodes.forEach((node) => node.texture.dispose());
    Object.values(hudMaterials).forEach((material) => material.dispose());
    bloomPass?.dispose();
    composer?.dispose();
    scene.traverse((object) => {
      const mesh = object as THREE.Mesh;
      if (mesh.geometry) {
        mesh.geometry.dispose();
      }
      const material = (mesh.material ?? null) as THREE.Material | THREE.Material[] | null;
      if (material) {
        if (Array.isArray(material)) {
          material.forEach((item) => item.dispose());
        } else {
          material.dispose();
        }
      }
    });
    renderer.dispose();
    if (renderer.domElement.parentElement) {
      renderer.domElement.parentElement.removeChild(renderer.domElement);
    }
  };
});

onBeforeUnmount(() => {
  if (cleanupScene) {
    cleanupScene();
    cleanupScene = null;
  }
});
</script>

<template>
  <div ref="containerRef" class="cyber-scene" aria-hidden="true"></div>
</template>
